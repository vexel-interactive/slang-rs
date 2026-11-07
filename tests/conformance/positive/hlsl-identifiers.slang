struct point
{
    float triangle;
};

RWStructuredBuffer<float> line;
RWStructuredBuffer<float> _slang_line;

point helper(float linear)
{
    point value;
    value.triangle = linear;
    return value;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    point value = helper(float(id.x));
    line[id.x] = value.triangle;
    _slang_line[id.x] = value.triangle + 1.0;
}
