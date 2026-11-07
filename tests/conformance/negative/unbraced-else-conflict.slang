float run()
{
    if (true)
        float value = 1.0;
    else
        float value = 2.0;
    return value;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
