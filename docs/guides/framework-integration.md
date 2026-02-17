# Framework Integration Guide

Cortex integrates with popular AI agent frameworks.

## LangChain

```python
from cortex_langchain import CortexMapTool, CortexQueryTool, CortexActTool
from langchain.agents import initialize_agent

tools = [CortexMapTool(), CortexQueryTool(), CortexActTool()]
agent = initialize_agent(tools, llm, agent="zero-shot-react-description")

result = agent.run("Map amazon.com and find products under $50")
```

## CrewAI

```python
from cortex_crewai import CortexWebCartographer
from crewai import Agent, Task, Crew

researcher = Agent(
    role="Web Researcher",
    tools=[CortexWebCartographer()],
)

task = Task(
    description="Map example.com and find all documentation pages",
    agent=researcher,
)

crew = Crew(agents=[researcher], tasks=[task])
crew.kickoff()
```

## OpenClaw

```json
{
  "skills": ["cortex_map", "cortex_navigate"],
  "task": "Map shop.example.com, find products with rating > 4.5"
}
```

```python
from openclaw import Agent
from cortex_openclaw.skills import cortex_map, cortex_navigate

agent = Agent(skills=[cortex_map, cortex_navigate])
result = agent.run("Find the cheapest laptop on shop.example.com")
```
