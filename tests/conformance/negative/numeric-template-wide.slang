RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    vector<float, 5> value;
    value[0] = 1.0;
    result[0] = value[0];
}
