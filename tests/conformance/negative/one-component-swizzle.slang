[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float1 value = 1;
    value.y = 2;
}
