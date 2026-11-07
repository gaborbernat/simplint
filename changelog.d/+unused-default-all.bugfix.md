`unused-default` (SMP506) follows the `__all__` of a library's public module, as the other API rules do: a public
function that `__all__` leaves out now has its default reported when every call in the run passes it.
