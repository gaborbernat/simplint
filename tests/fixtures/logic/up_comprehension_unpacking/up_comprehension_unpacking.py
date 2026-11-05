groups = [[1, 2], [3]]
pairs = [{"a": 1}, {"b": 2}]
flat_list = [x for group in groups for x in group]
flat_set = {x for group in groups for x in group}
flat_gen = (x for group in groups for x in group)
flat_call = list(x for group in groups for x in group)
parenthesized = [(x) for group in groups for x in group]
merged = {key: value for entry in pairs for key, value in entry.items()}
kept = [*group for group in groups]
unpacked_all = {**entry for entry in pairs}
filtered = [x for group in groups for x in group if x]
outer_filtered = [x for group in groups if group for x in group]
other_source = [x for group in groups for x in other]
renamed = [y for group in groups for x in group]
expression_element = [x + 1 for group in groups for x in group]
tuple_target = [x for group in groups for (x, y) in group]
self_named = [x for group in groups for group in group]
tuple_outer = [x for (a, b) in pairs for x in a]
single_generator = [x for x in groups]
keys_only = {key: value for entry in pairs for key, value in entry.keys()}
with_args = {key: value for entry in pairs for key, value in entry.items(1)}
with_keyword = {key: value for entry in pairs for key, value in entry.items(x=1)}
plain_items = {key: value for entry in pairs for key, value in items(entry)}
other_items = {key: value for entry in pairs for key, value in groups.items()}
mismatched_keys = {key: value for entry in pairs for other, value in entry.items()}
mismatched_values = {key: value for entry in pairs for key, other in entry.items()}
same_names = {key: key for entry in pairs for key, key in entry.items()}
single_target = {key: value for entry in pairs for pair in entry.items()}
triple_target = {key: value for entry in pairs for key, value, extra in entry.items()}
key_expression = {key + "": value for entry in pairs for key, value in entry.items()}
value_expression = {key: value + 1 for entry in pairs for key, value in entry.items()}
repeated_pairs = {key: value for entry in pairs for key, value in entry.items()}


async def gather(groups):
    flat = [x for group in groups for x in group]
    awaited = [x async for group in groups async for x in group]
    return flat, awaited
