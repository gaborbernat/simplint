from . import settings

configured = settings.set("a", 1)
print(set([1, 2]), configured)
