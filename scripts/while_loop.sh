COUNTER=-5
while [[ $COUNTER -lt 5 ]]; do
    echo The counter is $COUNTER
    let COUNTER=COUNTER+1 
done