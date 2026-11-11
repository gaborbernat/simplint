Finding the exception classes of the project follows each class to its subclasses once, instead of passing over every
class for each level of inheritance. A chain of 16,000 modules, each subclassing the exception of the one before, takes
0.96 seconds instead of 5.2.
