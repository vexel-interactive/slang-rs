[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float1 value = 1;
    float1x1 matrix = value;
}
