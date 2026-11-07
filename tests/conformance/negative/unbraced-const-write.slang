float run()
{
    if (true)
        const float value = 1.0;
    value = 2.0;
    return value;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
