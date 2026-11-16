interface Read { int read(double input); }
struct Value : Read {
    int read(int input) { return 11; }
    int read(float input) { return 22; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
