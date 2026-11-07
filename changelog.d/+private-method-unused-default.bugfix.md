`unused-default` (SMP506) again judges the private methods of a public class in a library's public module. Only the
public methods of such a class are public API, so the defaults of its private methods are checked.
