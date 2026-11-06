from typing import TYPE_CHECKING

from import_cycle_partner import partner_value

RUNTIME_FLAG = True
if TYPE_CHECKING:
    from import_cycle_partner import PartnerValue
elif RUNTIME_FLAG:
    from import_cycle_partner import partner_value as runtime_value


def later():
    from import_cycle_partner import total

    return total()


def typed(value: "PartnerValue") -> int:
    return partner_value() + value
