struct Marker { int value; __init<T>(T ignored) { value = 7; } };
int run() { return Marker.__init<int>(3).value; }
