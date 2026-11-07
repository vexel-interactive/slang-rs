struct Pair
{
    float value;
};

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair value = Pair(1, 2);
}
