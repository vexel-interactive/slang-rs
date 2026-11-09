struct Ops { void assign<T>(out T result, T value) { result = value; } };
void run() { Ops ops = {}; ops.assign(1, 2); }
