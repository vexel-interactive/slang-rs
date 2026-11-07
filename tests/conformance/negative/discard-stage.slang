void reject()
{
    discard;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    reject();
}
