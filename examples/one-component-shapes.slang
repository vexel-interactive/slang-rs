StructuredBuffer<float> inputs;
RWStructuredBuffer<float> results;

float choose(float value) { return value; }
float choose(float1 value) { return value[0] * 2.0; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    float1 value = float1(inputs[index]);
    vector<float, 1> doubled = value + value;
    float1x3 row = float1x3(1, 2, 3);
    matrix<float, 3, 1> column = { { 4 }, { 5 }, { 6 } };
    float1x1 product = mul(row, column);
    float1 sine, cosine;
    sincos(doubled, sine, cosine);
    results[index] = choose(value) + choose(row[0].x) + product[0][0] + determinant(product)
        + sine.x + cosine[0];
}
