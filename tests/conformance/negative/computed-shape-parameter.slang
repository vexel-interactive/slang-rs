void read(int Count) { vector<float,Count> value; }
[shader("compute")]
[numthreads(1,1,1)]
void main() { read(3); }
