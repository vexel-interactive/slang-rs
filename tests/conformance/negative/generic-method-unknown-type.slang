struct Ops { T copy<T>(T value) { return value; } };
int run() { Ops ops = {}; return ops.copy<Missing>(2); }
