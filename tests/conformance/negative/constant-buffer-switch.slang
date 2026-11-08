ConstantBuffer<int> input;
[shader("compute")]
[numthreads(1,1,1)]
void main() { switch(input) { case 0: break; default: break; } }
