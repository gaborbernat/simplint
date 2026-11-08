`single-user-constant` no longer moves a module variable that another file assigns or deletes as an attribute of its
module, as `module.NAME = value` does, which the move would silently break. `module-attribute-import` leaves only the
attribute another file stores to or a dotted string patches alone, as it does for `setattr`, so the module's other
attributes are still reported.
