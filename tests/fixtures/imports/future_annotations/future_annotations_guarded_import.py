from __future__ import annotations

MYPY = False
if MYPY:
    from decimal import Decimal


def price(amount: Decimal) -> Decimal:
    return amount
