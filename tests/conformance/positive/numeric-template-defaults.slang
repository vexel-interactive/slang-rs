static vector first = vector<>(1.0);
static vector<> second = vector<>(2.0);
static vector<double> third = vector<double>(3.0);
static matrix fourth = matrix<>(4.0);
static matrix<float, 2> fifth = matrix<float, 2>(5.0);

StructuredBuffer<vector<int, 0x2>> inputValues;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    vector<float, 2> values[2] = vector<float, 2>[2](
        vector<float, 2>(1.0), vector<float, 2>(2.0)
    );
    result[0] = first.x + second.y + float(third.z)
        + fourth[0][0] + fifth[1][2] + values[1].x
        + float(inputValues[0].x);
}
