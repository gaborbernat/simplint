import datetime
from datetime import datetime as moment


def module_form(text):
    return datetime.datetime.strptime(text, "%Y-%m-%d").date()


def named_form(text):
    return moment.strptime(text, "%H:%M").time()


def zone_form(text):
    return moment.strptime(text, "%H:%M %z").time()


def variable_format(text, fmt):
    return moment.strptime(text, fmt).time()


def keyword_format(text):
    return (moment.strptime(text, format="%Y").date())


def with_argument(text):
    return moment.strptime(text, "%Y").date(1)


def other_part(text):
    return moment.strptime(text, "%Y").timetz()


def no_parse(value):
    return value.date()


def not_a_call(value):
    return value.date


def other_strptime(text):
    return other.strptime(text, "%Y").date()


def plain_call(text):
    return moment(text).date()


def unnamed_format(text):
    return moment.strptime(text).date()


def name_only(text):
    return date_string(text).time()


def inherited(text):
    return moment.strptime(text, "%Y").date(), moment.strptime(text, "%Y")
