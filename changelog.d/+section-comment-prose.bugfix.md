`section-comment` (SMP001) no longer deletes the text of a decorated comment with more than three words, such as
`#  *** remove return to update cron`: its fix strips the decoration and keeps the text. Dividers, and banners around a
title of up to three words such as `# === helpers ===`, are still deleted. A pair of divider characters joined to a
word, such as the underscores of `__init__`, no longer makes a comment a section comment.
