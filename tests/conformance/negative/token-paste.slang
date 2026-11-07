#define CAT(a, b) a ## b

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    CAT(+, *)
}
