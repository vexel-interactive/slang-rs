T pick<T>(T first, T second) { return first; }
int run() { return pick(int2(1, 2), int3(3, 4, 5)).x; }
