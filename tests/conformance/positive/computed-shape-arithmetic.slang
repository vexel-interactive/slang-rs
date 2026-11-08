typedef vector<float,1+2> Value;
typedef row_major matrix<float,2,(1<<1)+1> Matrix;
ConstantBuffer<Value> input;
ConstantBuffer<Matrix> matrixValue;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    const int Count = 2;
    vector<float,Count> local = 1;
    vector<float,1<2?3:2> selected = vector<float,1<2?3:2>(1,2,3);
    Matrix matrix = (matrix<float,2,(1<<1)+1>)matrixValue;
    output[0] = local.y + selected.z + input.x + matrix[1][2];
}
