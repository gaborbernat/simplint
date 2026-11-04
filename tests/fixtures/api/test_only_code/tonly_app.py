from tonly_builders import TonlyShared, TonlyUsed, shared_tonly


def run() -> int:
    return shared_tonly() + len([TonlyShared, TonlyUsed])
