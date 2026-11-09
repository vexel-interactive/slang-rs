T copy<T>(T value) { return value; }
vector<T, 2> copy<T>(vector<T, 2> value) { return value; }
int run() { return copy(int2(2, 3)).x; }
