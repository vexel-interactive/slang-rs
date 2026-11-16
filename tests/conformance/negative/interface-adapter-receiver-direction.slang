interface Read { int read(float input); }
struct Value : Read {
    int count;
    [mutating] int read(int input) { count += input; return count; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
