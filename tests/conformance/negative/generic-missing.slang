struct Box<T>{T value;};[shader("compute")][numthreads(1,1,1)]void main(){Box<> value;}
