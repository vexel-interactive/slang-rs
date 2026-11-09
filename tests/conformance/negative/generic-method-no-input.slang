struct Ops { T create<T>() { return T(); } };
int run() { Ops ops = {}; return ops.create(); }
