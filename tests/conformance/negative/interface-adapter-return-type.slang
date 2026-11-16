struct Result { int value; };
interface Read { int read(float input); }
struct Value : Read {
    Result read(int input) { return Result(input); }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
