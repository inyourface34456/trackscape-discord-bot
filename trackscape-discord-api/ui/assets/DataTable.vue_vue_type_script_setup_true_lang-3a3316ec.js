import{d as k,r as w,c as b,b as e,g as x,v as F,u as o,i as g,n as f,h as l,F as d,j as i,o as t,t as y,z as v,k as C}from"./index-549232a9.js";const D={class:"table"},N={key:0,class:"text-center min-w-full"},S=l("td",null,[l("p",null,"No results found")],-1),T=[S],q=k({__name:"DataTable",props:{data:{type:Array,required:!0},columns:{type:Array,required:!0},searchField:{type:String,required:!1,default:""}},setup(_){const a=_;let s=w(""),c=b(()=>a.data.filter(n=>a.searchField===""?!0:n[a.searchField].toLowerCase().includes(s.value.toLowerCase())));return(n,m)=>(t(),e("div",null,[a.searchField!==""?x((t(),e("input",{key:0,"onUpdate:modelValue":m[0]||(m[0]=r=>g(s)?s.value=r:s=r),type:"text",placeholder:"Search",class:"input input-bordered w-full md:max-w-md max-w-full mb-3"},null,512)),[[F,o(s)]]):f("",!0),l("table",D,[l("thead",null,[l("tr",null,[(t(!0),e(d,null,i(a.columns,(r,u)=>(t(),e("th",{key:u},y(r.name),1))),128))])]),l("tbody",null,[o(c).length===0?(t(),e("tr",N,T)):f("",!0),(t(!0),e(d,null,i(o(c),(r,u)=>(t(),e("tr",{key:u},[(t(!0),e(d,null,i(a.columns,(p,h)=>(t(),e("th",{key:h},[v(n.$slots,"row-item",{column:p,item:r,index:h},()=>[C(y(r[p.key]),1)])]))),128))]))),128))])])]))}});export{q as _};