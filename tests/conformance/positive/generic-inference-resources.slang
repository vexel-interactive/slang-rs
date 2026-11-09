T copy<T>(T value) { return T(value); }
StructuredBuffer<T> structured<T>(StructuredBuffer<T> value) { return value; }
ConstantBuffer<T> uniform<T>(ConstantBuffer<T> value) { return value; }
vector<T, 2> shaped<T>(vector<T, 2> value) { return value; }
T arrayElement<T>(T value[2]) { return value[1]; }
struct Item { int value; };
ConstantBuffer<Item> item;
ConstantBuffer<float2> numeric;
ConstantBuffer<int[2]> arrayInput;
StructuredBuffer<int> input;
StructuredBuffer<row_major float2x2> matrices;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(copy(item).value);
    output[1] = uint(uniform(item).value);
    output[2] = uint(structured(input)[0]);
    output[3] = uint(shaped(numeric).y);
    output[4] = uint(copy(matrices)[0][1][1]);
    output[5] = uint(arrayElement(arrayInput));
}
