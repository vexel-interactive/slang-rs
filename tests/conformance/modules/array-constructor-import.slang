import "initializer-lib.slang";
typedef Pair Values[2];
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Values values = Values({1.0});
    Values copy = Pair[2](values);
    result[0] = copy[0].x + copy[1].x;
}
