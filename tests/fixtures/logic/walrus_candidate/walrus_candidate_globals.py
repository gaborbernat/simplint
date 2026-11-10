import os

process_count = os.cpu_count()
if process_count:
    print(process_count)
print(globals())
