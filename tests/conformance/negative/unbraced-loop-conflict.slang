void run()
{
    for (int i = 0; i < 1; i++)
        int i = 2;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
