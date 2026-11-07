RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { switch(2) { case (1, 2): result[0] = 1.0; break; } }
