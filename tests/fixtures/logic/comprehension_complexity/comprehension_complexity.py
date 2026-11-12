ROWS = [[1, 2], [3, 4]]

FLAT = [cell for row in ROWS for cell in row]
TRIPLE = {(a, b, c) for a in ROWS for b in a for c in range(b)}
GENERATOR = sum(cell for row in ROWS for cell in row)
PAIRS = {cell: row for row in ROWS for cell in row}
FILTERED = [row for row in ROWS if row if len(row) > 1 if row[0]]

ONE_FILTER = [row for row in ROWS if row and len(row) > 1]
SIMPLE = {len(row): row for row in ROWS}
