struct Ops { int value; [mutating] T copy<T>(T input) { value++; return input; } };
int run() { return Ops(2).copy<int>(3); }
