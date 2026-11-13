def unpacked_dict_comprehension(parts, text: str):
    merged = {**part for part in parts}
    return text[:2] == "ab", merged
