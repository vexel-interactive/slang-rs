static const float fractional = (float)1.5;
static const int converted = (int)(fractional * 2);
RWStructuredBuffer<int> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    results[0] = converted;
}
