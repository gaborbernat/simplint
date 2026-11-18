lazy import ineffective_lazy_declaration_pkg
lazy from ineffective_lazy_declaration_pkg import helper


def use() -> object:
    return ineffective_lazy_declaration_pkg, helper
