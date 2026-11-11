def run(items, prepare, log, send):
    # Retries allowed before giving up: the upstream API drops one call in three.
    limit = 3
    prepare(items)
    log(items)
    return send(items, limit)


def plain(items, prepare, log, send):
    limit = 3
    prepare(items)
    log(items)
    return send(items, limit)
