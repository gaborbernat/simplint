import datetime
from datetime import datetime as moment


def module_form(text):
    return datetime.datetime.strptime(text, "%Y-%m-%d").date()


def named_form(text):
    return moment.strptime(text, "%H:%M").time()


def zone_form(text):
    return moment.strptime(text, "%H:%M %z").time()


def colon_zone_form(text):
    return datetime.datetime.strptime(text, "%H:%M %:z").time()


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


def commented_time(text):
    return moment.strptime(text, "%H:%M").time(  # keep the call
    )


def leap_day_time(text):
    return moment.strptime(text, "%m-%d %H:%M").time()


def time_of_day_format(text):
    return moment.strptime(text, "%H:%M.%f %p %%").time()


def time_with_seconds(text):
    return moment.strptime(text, "%H:%M:%S").time()


def date_with_seconds(text):
    return moment.strptime(text, "%Y-%m-%d %H:%M:%S").date()


def date_with_composite_seconds(text):
    return moment.strptime(text, "%Y-%m-%d %X").date()


def date_with_literal_percent(text):
    return moment.strptime(text, "%Y-%m-%d %%S").date()


def date_variable_format(text, fmt):
    return moment.strptime(text, fmt).date()
