struct Box<T>{vector<T,2> value;[mutating]void write(vector<T,2> other){value.xx=other;}};[shader("compute")][numthreads(1,1,1)]void main(){}
