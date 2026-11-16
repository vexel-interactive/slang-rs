interface Read { int read(float input, out int value); }
struct Value : Read {
    int read(int input, inout int value) { value = 3; return input; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
