ByteAddressBuffer input;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    input.Store<float>(0, 1);
}
