Collecting the fields of a `@dataclass` checks each new field against a set instead of the fields before it, so a
dataclass with 64,000 fields takes 0.06 seconds instead of 3.9.
