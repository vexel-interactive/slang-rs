import lib;
[shader("compute")][numthreads(1,1,1)] void main() { Item x = Item(2); x.set(3); }
