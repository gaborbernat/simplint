def show(items):
    for i in range(len(items)):
        print(items[i])


def attribute(self):
    for i in range(len(self.names)):
        print(self.names[i])


def position_used(items):
    for i in range(len(items)):
        print(i, items[i])


def written(items):
    for i in range(len(items)):
        items[i] = items[i].strip()


def index_after(items):
    for i in range(len(items)):
        if items[i]:
            break
    return i


def name_taken(rows):
    row = None
    for i in range(len(rows)):
        print(rows[i], row)


def other_sequence(items, labels):
    for i in range(len(items)):
        print(labels[i])


def index_only(items):
    for i in range(len(items)):
        print(i)


def call_result(load):
    for i in range(len(load())):
        print(load()[i])


def counted(items, count):
    for i in range(count(items)):
        print(items[i])
