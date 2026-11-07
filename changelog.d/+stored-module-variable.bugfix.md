`single-user-constant` no longer moves a module variable that another file assigns or deletes as an attribute of its
module, as `module.NAME = value` does, which the move would silently break. `module-attribute-import` leaves a module
alone when another file stores to one of its attributes, as it does for `setattr`.
