interface Read { int read(float input, out int value); }
struct Value : Read {
    int read(int input, out float value) { value = 3; return input; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
