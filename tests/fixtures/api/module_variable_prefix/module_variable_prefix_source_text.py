limit = 3
template = t"{limit}"
print(template.interpolations[0].expression, f"{limit=}")
