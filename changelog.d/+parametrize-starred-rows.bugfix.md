`parametrize-without-id` (SMP802) no longer reports a starred generator in a parametrize list, such as
`*(pytest.param(...) for ...)`, since the rows it unpacks are not known. A starred list or tuple, written inline or
bound to a name, has its rows checked instead.
