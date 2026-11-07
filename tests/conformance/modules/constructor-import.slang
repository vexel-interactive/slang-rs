import constructor_lib;

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair value = Pair(2.0);
    Pair copy = Pair(value);
    result[0] = copy.value;
}
