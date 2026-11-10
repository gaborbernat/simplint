[`single-user-constant`](../src/rules/layout/single_user_constant.md) keeps a list, set or dict at module level when the
function returns it, passes it to another function or binds another name to it, since a fresh copy on each call would
lose the state other code changed.
