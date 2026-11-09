struct Marker { int value; __init<T>(T ignored) { value = 7; } };
int run() { return Marker(3, 4).value; }
