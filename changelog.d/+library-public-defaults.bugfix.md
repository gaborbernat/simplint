`unused-default` (SMP506) no longer removes a default from a public function or class of a library's public module,
since code outside the project may call it without the argument. Its hooks and entry points were already left alone.
