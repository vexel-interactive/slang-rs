struct Ops { T copy<T>(T value) { return value; } };
int run() { return Ops.copy<int>(2); }
