interface Read {
    int read(float input) { return 3; }
    int read(double input) { return 7; }
}
struct Value : Read {
    int read(int input) { return 11; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
