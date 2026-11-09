struct Box<T> { T value; };
typedef int Integers[2];
typealias VectorAlias = vector<int, 2>;
static const int width = 1 + 1;
int readArray(Box<int[2]> input) { return input.value[1]; }
int readVector(Box<int2> input) { return input.value.y; }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    const int localWidth = 2;
    Box<Integers> first = {{2, 3}};
    Box<int[1 + 1]> second = {{4, 5}};
    Box<int[localWidth]> third = {{6, 7}};
    Box<vector<int, width>> fourth = {{8, 9}};
    Box<VectorAlias> fifth = {{10, 11}};
    output[0] = uint(readArray(first));
    output[1] = uint(readArray(second));
    output[2] = uint(readArray(third));
    output[3] = uint(readVector(fourth));
    output[4] = uint(readVector(fifth));
}
