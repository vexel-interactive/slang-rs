struct Ops { int value; [mutating] T copy<T>(T input) { value++; return input; } };
int run() { const Ops ops = {2}; return ops.copy<int>(3); }
