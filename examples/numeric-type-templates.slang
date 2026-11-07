typealias Direction = vector<float, 3>;
typealias Basis = matrix<float, 3, 3>;

StructuredBuffer<Direction> inputValues;
RWStructuredBuffer<Direction> outputValues;

[shader("compute")]
[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    Basis basis = matrix<float, 3, 3>(
        vector<float, 3>(1.0, 0.0, 0.0),
        vector<float, 3>(0.0, 2.0, 0.0),
        vector<float, 3>(0.0, 0.0, 3.0)
    );
    Direction bias = vector<float, 3>(0.5);
    outputValues[id.x] = mul(basis, inputValues[id.x]) + bias;
}
