use ruff_python_ast::{self as ast, Expr, Operator};

// A value the compiler can hold as a constant, and the shape it needs to decide whether a later operation folds. A
// binary operation on constants folds only when CPython computes it at compile time, so an inlined expression costs
// nothing per call; a result beyond these limits is left to run on each call.
#[derive(Clone, Copy)]
enum Folded {
    Int(i128),
    Float(f64),
    Complex,
    Str(usize),
    Bytes(usize),
    // A tuple's length and the items its tuples hold, which CPython counts against `MAX_TOTAL_ITEMS` when repeated.
    Tuple { len: usize, nested: usize },
    // An integer too large to model, which may be a constant but whose value this model does not know.
    Opaque,
    // `None` and `...`, which no arithmetic accepts.
    Other,
}

// The limits of CPython's constant folding, taken from the oldest version tox tests, 3.11: the constants at
// Python/ast_opt.c:147-150, the guards in safe_multiply, safe_power, safe_lshift and safe_mod at lines 152-238, and the
// binary operations that apply them at lines 478-493. The values and guards are the same in 3.12 and 3.13 (ast_opt.c)
// and in 3.14 and 3.15 (Python/flowgraph.c:1713-1716 and 1789-1792), so one rule serves every version tox tests.
// `safe_mod` refuses a string or bytes left operand, so `%` formatting is never folded here; 3.11.18 and 3.15.2 both
// leave `'%s-%s' % (1, 2)` as a BINARY_OP.
const MAX_INT_SIZE: u32 = 128;
const MAX_COLLECTION_SIZE: usize = 256;
const MAX_STR_SIZE: usize = 4096;
const MAX_TOTAL_ITEMS: usize = 1024;

// Whether the value is a constant the compiler stores with the code, so that writing it out in a function costs nothing
// per call: a literal, a tuple of them, or a binary operation the compiler folds.
pub(super) fn is_literal(value: &Expr) -> bool {
    folded(value).is_some()
}

fn folded(value: &Expr) -> Option<Folded> {
    match value {
        Expr::NumberLiteral(ast::ExprNumberLiteral { value, .. }) => Some(match value {
            ast::Number::Int(int) => int.as_u64().map_or(Folded::Opaque, |small| Folded::Int(i128::from(small))),
            ast::Number::Float(float) => Folded::Float(*float),
            ast::Number::Complex { .. } => Folded::Complex,
        }),
        Expr::BooleanLiteral(ast::ExprBooleanLiteral { value, .. }) => Some(Folded::Int(i128::from(*value))),
        Expr::NoneLiteral(_) | Expr::EllipsisLiteral(_) => Some(Folded::Other),
        Expr::StringLiteral(literal) => Some(Folded::Str(literal.value.to_str().chars().count())),
        Expr::BytesLiteral(literal) => Some(Folded::Bytes(literal.value.len())),
        Expr::Tuple(ast::ExprTuple { elts, .. }) => {
            elts.iter().map(folded).collect::<Option<Vec<_>>>().map(|items| tuple_of(&items))
        }
        Expr::UnaryOp(ast::ExprUnaryOp { op: ast::UnaryOp::USub, operand, .. }) => negate(folded(operand)?),
        Expr::BinOp(ast::ExprBinOp { left, op, right, .. }) => binary(*op, folded(left)?, folded(right)?),
        _ => None,
    }
}

fn tuple_of(items: &[Folded]) -> Folded {
    let nested = items
        .iter()
        .map(|item| match item {
            Folded::Tuple { len, nested } => len + nested,
            _ => 0,
        })
        .sum();
    Folded::Tuple { len: items.len(), nested }
}

fn negate(value: Folded) -> Option<Folded> {
    match value {
        Folded::Int(int) => int.checked_neg().map(Folded::Int),
        Folded::Float(float) => Some(Folded::Float(-float)),
        Folded::Complex => Some(Folded::Complex),
        Folded::Opaque => Some(Folded::Opaque),
        Folded::Str(_) | Folded::Bytes(_) | Folded::Tuple { .. } | Folded::Other => None,
    }
}

fn binary(op: Operator, left: Folded, right: Folded) -> Option<Folded> {
    match (left, right) {
        (Folded::Int(left), Folded::Int(right)) => int_binary(op, left, right),
        (Folded::Int(left), Folded::Float(right)) => float_binary(op, to_float(left), right),
        (Folded::Float(left), Folded::Int(right)) => float_binary(op, left, to_float(right)),
        (Folded::Float(left), Folded::Float(right)) => float_binary(op, left, right),
        (Folded::Complex, other) | (other, Folded::Complex) if complex_operand(other) => match op {
            Operator::Add | Operator::Sub | Operator::Mult => Some(Folded::Complex),
            Operator::Div => match right {
                Folded::Int(divisor) if divisor != 0 => Some(Folded::Complex),
                Folded::Float(divisor) if divisor != 0.0 => Some(Folded::Complex),
                _ => None,
            },
            _ => None,
        },
        (Folded::Str(left), Folded::Str(right)) if op == Operator::Add => left.checked_add(right).map(Folded::Str),
        (Folded::Bytes(left), Folded::Bytes(right)) if op == Operator::Add => {
            left.checked_add(right).map(Folded::Bytes)
        }
        (
            Folded::Tuple { len: left_len, nested: left_nested },
            Folded::Tuple { len: right_len, nested: right_nested },
        ) if op == Operator::Add => {
            Some(Folded::Tuple { len: left_len + right_len, nested: left_nested + right_nested })
        }
        (Folded::Str(size), Folded::Int(count)) | (Folded::Int(count), Folded::Str(size)) if op == Operator::Mult => {
            repeat_string(count, size, MAX_STR_SIZE).map(Folded::Str)
        }
        (Folded::Bytes(size), Folded::Int(count)) | (Folded::Int(count), Folded::Bytes(size))
            if op == Operator::Mult =>
        {
            repeat_string(count, size, MAX_STR_SIZE).map(Folded::Bytes)
        }
        (Folded::Tuple { len, nested }, Folded::Int(count)) | (Folded::Int(count), Folded::Tuple { len, nested })
            if op == Operator::Mult =>
        {
            repeat_tuple(count, len, nested)
        }
        _ => None,
    }
}

fn complex_operand(value: Folded) -> bool {
    matches!(value, Folded::Int(_) | Folded::Float(_) | Folded::Complex | Folded::Opaque)
}

// `safe_multiply` checks a string or bytes repetition only when the string is not empty: the result must not exceed
// MAX_STR_SIZE characters. A negative count is refused for a non-empty string, as the guard returns before the call.
fn repeat_string(count: i128, size: usize, limit: usize) -> Option<usize> {
    if size == 0 {
        return repeat_empty(count).then_some(0);
    }
    let count = usize::try_from(count).ok()?;
    count.checked_mul(size).filter(|total| *total <= limit)
}

// Repeating an empty sequence still converts the count to a `Py_ssize_t`, which raises for a count beyond it. The
// bound is 32 bits, the smallest index size of the platforms, so the result does not depend on the build.
fn repeat_empty(count: i128) -> bool {
    count.unsigned_abs() <= u128::from(i32::MAX.unsigned_abs())
}

// `safe_multiply` checks a tuple repetition by its item count, and by the items the tuple holds counted through its
// nested tuples when the count is not zero (`check_complexity`).
fn repeat_tuple(count: i128, len: usize, nested: usize) -> Option<Folded> {
    if len == 0 {
        return repeat_empty(count).then_some(Folded::Tuple { len: 0, nested: 0 });
    }
    let count = usize::try_from(count).ok()?;
    if count == 0 {
        return Some(Folded::Tuple { len: 0, nested: 0 });
    }
    if count.checked_mul(len)? > MAX_COLLECTION_SIZE || count.checked_mul(len + nested)? > MAX_TOTAL_ITEMS {
        return None;
    }
    Some(Folded::Tuple { len: count * len, nested: count * nested })
}

// `const_folding_safe_power` and `safe_power` refuse an integer power whose base's bit length times the exponent exceeds
// MAX_INT_SIZE, and a non-zero base to a positive exponent is the only case they check.
fn int_binary(op: Operator, left: i128, right: i128) -> Option<Folded> {
    let int = |value: Option<i128>| value.map(Folded::Int);
    match op {
        Operator::Add => int(left.checked_add(right)),
        Operator::Sub => int(left.checked_sub(right)),
        Operator::Mult => {
            if left != 0 && right != 0 && bit_length(left) + bit_length(right) > MAX_INT_SIZE {
                return None;
            }
            int(left.checked_mul(right))
        }
        Operator::Div => (right != 0).then(|| Folded::Float(to_float(left) / to_float(right))),
        Operator::FloorDiv => int(floor_div(left, right)),
        Operator::Mod => int(floor_mod(left, right)),
        Operator::Pow => int_power(left, right),
        Operator::LShift => int_shift_left(left, right),
        Operator::RShift => (right >= 0).then(|| {
            Folded::Int(if right >= i128::from(MAX_INT_SIZE) { if left < 0 { -1 } else { 0 } } else { left >> right })
        }),
        Operator::BitOr => int(Some(left | right)),
        Operator::BitXor => int(Some(left ^ right)),
        Operator::BitAnd => int(Some(left & right)),
        Operator::MatMult => None,
    }
}

fn int_power(base: i128, exponent: i128) -> Option<Folded> {
    if base == 0 {
        return match exponent {
            0 => Some(Folded::Int(1)),
            1.. => Some(Folded::Int(0)),
            _ => None,
        };
    }
    if exponent < 0 {
        return float_result(to_float(base).powf(to_float(exponent)));
    }
    if exponent == 0 {
        return Some(Folded::Int(1));
    }
    if i128::from(bit_length(base)) > i128::from(MAX_INT_SIZE) / exponent {
        return None;
    }
    base.checked_pow(u32::try_from(exponent).ok()?).map(Folded::Int)
}

fn int_shift_left(value: i128, count: i128) -> Option<Folded> {
    if count < 0 {
        return None;
    }
    if value == 0 {
        return Some(Folded::Int(0));
    }
    if count == 0 {
        return Some(Folded::Int(value));
    }
    if count > i128::from(MAX_INT_SIZE) || i128::from(bit_length(value)) + count > i128::from(MAX_INT_SIZE) {
        return None;
    }
    let magnitude = value.unsigned_abs() << count;
    let magnitude = i128::try_from(magnitude).ok()?;
    Some(Folded::Int(if value < 0 { -magnitude } else { magnitude }))
}

fn floor_div(left: i128, right: i128) -> Option<i128> {
    let quotient = left.checked_div(right)?;
    let remainder = left.checked_rem(right)?;
    if remainder != 0 && (remainder < 0) != (right < 0) { quotient.checked_sub(1) } else { Some(quotient) }
}

fn floor_mod(left: i128, right: i128) -> Option<i128> {
    let remainder = left.checked_rem(right)?;
    if remainder != 0 && (remainder < 0) != (right < 0) { remainder.checked_add(right) } else { Some(remainder) }
}

fn bit_length(value: i128) -> u32 {
    value.unsigned_abs().bit_width()
}

fn float_binary(op: Operator, left: f64, right: f64) -> Option<Folded> {
    match op {
        Operator::Add => Some(Folded::Float(left + right)),
        Operator::Sub => Some(Folded::Float(left - right)),
        Operator::Mult => Some(Folded::Float(left * right)),
        Operator::Div => (right != 0.0).then(|| Folded::Float(left / right)),
        Operator::FloorDiv => (right != 0.0).then(|| Folded::Float((left / right).floor())),
        Operator::Mod => (right != 0.0).then(|| Folded::Float(float_mod(left, right))),
        Operator::Pow => float_power(left, right),
        Operator::LShift
        | Operator::RShift
        | Operator::BitOr
        | Operator::BitXor
        | Operator::BitAnd
        | Operator::MatMult => None,
    }
}

// A float power that CPython computes: a zero base to a negative power raises, a negative base to a fractional power is
// complex, and an overflowing result raises, so each of those is left out.
fn float_power(base: f64, exponent: f64) -> Option<Folded> {
    if (base == 0.0 && exponent < 0.0) || (base < 0.0 && exponent.fract() != 0.0) {
        return None;
    }
    float_result(base.powf(exponent))
}

// The float nearest the integer, which is how CPython converts an integer to a float in true division and in mixed
// arithmetic. Rust's decimal parser rounds the same way, so no `as` cast loses bits silently.
fn to_float(value: i128) -> f64 {
    value.to_string().parse().unwrap_or(f64::NAN)
}

fn float_mod(left: f64, right: f64) -> f64 {
    let remainder = left % right;
    if remainder != 0.0 && (remainder < 0.0) != (right < 0.0) { remainder + right } else { remainder }
}

fn float_result(value: f64) -> Option<Folded> {
    value.is_finite().then_some(Folded::Float(value))
}
