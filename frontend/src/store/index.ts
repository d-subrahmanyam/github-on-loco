import { configureStore } from '@reduxjs/toolkit'
import createSagaMiddleware from 'redux-saga'
import browseReducer from './browseSlice'
import collaboratorsReducer from './collaboratorsSlice'
import issuesReducer from './issuesSlice'
import pullsReducer from './pullsSlice'
import repositoriesReducer from './repositoriesSlice'
import rootSaga from './sagas'
import socialReducer from './socialSlice'
import tokensReducer from './tokensSlice'

/** Build a fresh store with its own saga runtime. Tests use this for isolation. */
export function createAppStore() {
  const sagaMiddleware = createSagaMiddleware()
  const store = configureStore({
    reducer: {
      repositories: repositoriesReducer,
      browse: browseReducer,
      collaborators: collaboratorsReducer,
      social: socialReducer,
      issues: issuesReducer,
      pulls: pullsReducer,
      tokens: tokensReducer,
    },
    // Saga owns side effects; the default async thunk middleware is not needed.
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({ thunk: false }).concat(sagaMiddleware),
  })
  sagaMiddleware.run(rootSaga)
  return store
}

/** The single store used by the app. */
export const store = createAppStore()

export type RootState = ReturnType<typeof store.getState>
export type AppDispatch = typeof store.dispatch
