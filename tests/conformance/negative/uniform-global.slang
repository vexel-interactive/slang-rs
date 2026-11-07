float value;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    value = 1.0;
}
